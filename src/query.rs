use serde_json::Value;

#[derive(Debug, Clone)]
pub struct WhereClause {
    pub column: String,
    pub operator: String,
    pub value: Value,
}

#[derive(Debug, Clone)]
pub struct OrderBy {
    pub column: String,
    pub ascending: bool,
}

#[derive(Debug)]
pub struct Query {
    pub table: String,
    pub where_clauses: Vec<WhereClause>,
    pub order_by: Vec<OrderBy>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Query {
    pub fn new(table: String) -> Self {
        Self {
            table,
            where_clauses: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    pub fn where_clause(&mut self, column: &str, operator: &str, value: Value) -> &mut Self {
        self.where_clauses.push(WhereClause {
            column: column.to_string(),
            operator: operator.to_string(),
            value,
        });
        self
    }

    pub fn order_by(&mut self, column: &str, ascending: bool) -> &mut Self {
        self.order_by.push(OrderBy {
            column: column.to_string(),
            ascending,
        });
        self
    }

    pub fn limit(&mut self, limit: i64) -> &mut Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(&mut self, offset: i64) -> &mut Self {
        self.offset = Some(offset);
        self
    }

    pub fn to_sql(&self) -> (String, Vec<Value>) {
        let mut sql = format!("SELECT * FROM {}", self.table);
        let mut params = Vec::new();

        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            for (idx, clause) in self.where_clauses.iter().enumerate() {
                if idx > 0 {
                    sql.push_str(" AND ");
                }
                sql.push_str(&format!("{} {} ?", clause.column, clause.operator));
                params.push(clause.value.clone());
            }
        }

        if !self.order_by.is_empty() {
            sql.push_str(" ORDER BY ");
            for (idx, order) in self.order_by.iter().enumerate() {
                if idx > 0 {
                    sql.push_str(", ");
                }
                sql.push_str(&format!("{} {}", order.column, if order.ascending { "ASC" } else { "DESC" }));
            }
        }

        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        (sql, params)
    }

    pub fn to_count_sql(&self) -> (String, Vec<Value>) {
        let mut sql = format!("SELECT COUNT(*) as count FROM {}", self.table);
        let mut params = Vec::new();

        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            for (idx, clause) in self.where_clauses.iter().enumerate() {
                if idx > 0 {
                    sql.push_str(" AND ");
                }
                sql.push_str(&format!("{} {} ?", clause.column, clause.operator));
                params.push(clause.value.clone());
            }
        }

        (sql, params)
    }
}

#[derive(Debug)]
pub struct InsertQuery {
    pub table: String,
    pub columns: Vec<String>,
    pub values: Vec<Value>,
}

impl InsertQuery {
    pub fn new(table: String) -> Self {
        Self {
            table,
            columns: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn set(&mut self, column: &str, value: Value) -> &mut Self {
        self.columns.push(column.to_string());
        self.values.push(value);
        self
    }

    pub fn to_sql(&self) -> (String, Vec<Value>) {
        let columns = self.columns.join(", ");
        let placeholders = vec!["?"; self.values.len()].join(", ");
        
        let sql = format!("INSERT INTO {} ({}) VALUES ({}) RETURNING *", 
                         self.table, columns, placeholders);
        
        (sql, self.values.clone())
    }
}

#[derive(Debug)]
pub struct UpdateQuery {
    pub table: String,
    pub columns: Vec<String>,
    pub values: Vec<Value>,
    pub where_clauses: Vec<WhereClause>,
}

impl UpdateQuery {
    pub fn new(table: String) -> Self {
        Self {
            table,
            columns: Vec::new(),
            values: Vec::new(),
            where_clauses: Vec::new(),
        }
    }

    pub fn set(&mut self, column: &str, value: Value) -> &mut Self {
        self.columns.push(column.to_string());
        self.values.push(value);
        self
    }

    pub fn where_clause(&mut self, column: &str, operator: &str, value: Value) -> &mut Self {
        self.where_clauses.push(WhereClause {
            column: column.to_string(),
            operator: operator.to_string(),
            value,
        });
        self
    }

    pub fn to_sql(&self) -> (String, Vec<Value>) {
        let mut sql = format!("UPDATE {} SET ", self.table);
        let mut params = self.values.clone();
        
        for (idx, column) in self.columns.iter().enumerate() {
            if idx > 0 {
                sql.push_str(", ");
            }
            sql.push_str(&format!("{} = ?", column));
        }

        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            for (idx, clause) in self.where_clauses.iter().enumerate() {
                if idx > 0 {
                    sql.push_str(" AND ");
                }
                sql.push_str(&format!("{} {} ?", clause.column, clause.operator));
                params.push(clause.value.clone());
            }
        }

        sql.push_str(" RETURNING *");

        (sql, params)
    }
}