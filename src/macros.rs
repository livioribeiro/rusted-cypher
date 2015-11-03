#[macro_export]
macro_rules! cypher_stmt {
    ( $s:expr ) => { $crate::Statement::new($s) };
    ( $s:expr { $( $k:expr => $v:expr ),+ } ) => {
        $crate::Statement::new($s)
            $(.with_param($k, $v))*
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[allow(unused_variables)]
    fn statement_without_params() {
        let statement = cypher_stmt!("MATCH n RETURN n");
    }

    #[test]
    fn statement_with_single_param() {
        let statement1 = cypher_stmt!("MATCH n RETURN n" {
            "name" => "test"
        });

        let param = 1;
        let statement2 = cypher_stmt!("MATCH n RETURN n" {
            "value" => param
        });

        assert_eq!("test", statement1.get_param::<String>("name").unwrap().unwrap());
        assert_eq!(param, statement2.get_param::<i32>("value").unwrap().unwrap());
    }

    #[test]
    fn statement_with_multiple_params() {
        let param = 3f32;
        let statement = cypher_stmt!("MATCH n RETURN n" {
            "param1" => "one",
            "param2" => 2,
            "param3" => param
        });

        assert_eq!("one", statement.get_param::<String>("param1").unwrap().unwrap());
        assert_eq!(2, statement.get_param::<i32>("param2").unwrap().unwrap());
        assert_eq!(param, statement.get_param::<f32>("param3").unwrap().unwrap());
    }
}
