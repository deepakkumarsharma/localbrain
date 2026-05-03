pub const CREATE_FILE_TABLE: &str = "
CREATE NODE TABLE IF NOT EXISTS File(
  path STRING PRIMARY KEY,
  language STRING,
  updated_at STRING
)";

pub const CREATE_SYMBOL_TABLE: &str = "
CREATE NODE TABLE IF NOT EXISTS Symbol(
  id STRING PRIMARY KEY,
  file_path STRING,
  name STRING,
  kind STRING,
  parent STRING,
  source STRING,
  start_line INT64,
  start_column INT64,
  end_line INT64,
  end_column INT64
)";

pub const CREATE_CONTAINS_TABLE: &str = "
CREATE REL TABLE IF NOT EXISTS CONTAINS(FROM File TO Symbol)
";
