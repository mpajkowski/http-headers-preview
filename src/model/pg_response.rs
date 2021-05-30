use tokio_postgres::SimpleQueryMessage;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Header {
    columns: Vec<String>,
}

impl Header {
    pub fn new(columns: Vec<String>) -> Self {
        Self { columns }
    }

    pub fn push<T: Into<String>>(&mut self, col: T) {
        self.columns.push(col.into());
    }

    /// Set the header's columns.
    pub fn set_columns(&mut self, columns: Vec<String>) {
        self.columns = columns;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Row {
    values: Vec<String>,
}

impl Row {
    pub fn new(values: Vec<String>) -> Self {
        Self { values }
    }

    pub fn push<T: Into<String>>(&mut self, value: T) {
        self.values.push(value.into());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Table {
    header: Header,
    rows: Vec<Row>,
}

impl Table {
    pub fn new(header: Header, rows: Vec<Row>) -> Self {
        Self { header, rows }
    }

    pub fn as_pretty(&self) -> prettytable::Table {
        let mut table = prettytable::Table::new();

        table.add_row(self.header.columns.clone().into());
        self.rows.iter().map(|row| &row.values).for_each(|row| {
            table.add_row(row.into());
        });

        table
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PgResponse {
    Table(Table),
    CommandComplete(u64),
}

impl PgResponse {
    pub fn process_batches(batches: Vec<SimpleQueryMessage>) -> Vec<PgResponse> {
        let mut responses = vec![];

        let mut table: Option<Table> = None;
        let mut prev_columns: Vec<String> = vec![];
        for batch in batches {
            match batch {
                SimpleQueryMessage::Row(row) => {
                    let current_table = table.get_or_insert_with(|| {
                        let mut table = Table::default();
                        let column_names = row
                            .columns()
                            .iter()
                            .map(|col| col.name().to_string())
                            .collect::<Vec<String>>();

                        prev_columns = column_names.clone();
                        table.header.columns = column_names;

                        table
                    });

                    let current_columns = row
                        .columns()
                        .iter()
                        .map(|c| c.name().to_string())
                        .collect::<Vec<_>>();

                    let row_values = (0..current_columns.len())
                        .map(|idx| row.get(idx).unwrap_or("[null]").to_string())
                        .collect::<Vec<_>>();

                    if current_columns == prev_columns {
                        current_table.rows.push(Row::new(row_values));
                    } else {
                        let ready_table = table.take().unwrap();
                        responses.push(PgResponse::Table(ready_table));

                        let header = Header::new(current_columns.clone());
                        table = Some(Table::new(header, vec![Row::new(row_values)]));
                        prev_columns = current_columns;
                    }
                }
                SimpleQueryMessage::CommandComplete(rows_affected) => {
                    if table.is_some() {
                        let ready_table = table.take().unwrap();
                        prev_columns = vec![];
                        responses.push(PgResponse::Table(ready_table));
                    }
                    responses.push(PgResponse::CommandComplete(rows_affected))
                }
                _ => unreachable!(),
            }
        }

        if let Some(table) = table {
            responses.push(PgResponse::Table(table));
        }

        responses
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cascade::cascade;

    #[test]
    fn prettytable() {
        let table = Table {
            header: cascade! {
                Header::default();
                ..push("col1");
                ..push("col2");
                ..push("col3");
            },
            rows: cascade! {
                vec![];
                ..push(cascade! {
                    Row::default();
                    ..push("value1_1");
                    ..push("value2_1");
                    ..push("value3_1");
                });
                ..push(cascade! {
                    Row::default();
                    ..push("value1_2");
                    ..push("value2_2");
                    ..push("value3_2");
                });
                ..push(cascade! {
                    Row::default();
                    ..push("value1_3");
                    ..push("value2_3");
                    ..push("value3_3");
                });
            },
        };

        let pretty = table.as_pretty();
        pretty.printstd();
    }
}