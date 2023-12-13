macro_rules! newtype_enum {
    { $v:vis enum $name:ident { $($case:ident,)+ } } => {
        $v enum $name {
            $($case,)+
        }

        ::paste::paste! {
            $v mod [< $name:snake:lower >] {
                $(
                    pub struct $case;
                )+
            }
        }
    };
    { #[derive $d:tt] $v:vis enum $name:ident { $($case:ident,)+ } } => {
        #[derive $d]
        $v enum $name {
            $($case,)+
        }

        ::paste::paste! {
            $v mod [< $name:snake:lower >] {
                $(
                    #[derive $d]
                    pub struct $case;
                )+
            }
        }
    };
    {
        #[derive $d:tt]
        #[derive $d2:tt]
        #[$extra:ident $e:tt]
        $v:vis enum $name:ident { $($case:ident,)+ }} => {
            #[derive $d]
            #[derive $d2]
            #[$extra $e]
            $v enum $name {
                $($case,)+
            }

            ::paste::paste! {
                $v mod [< $name:snake:lower >] {
                    $(
                        #[derive $d]
                        pub struct $case;
                    )+
                }
            }
    };
}

pub(crate) use newtype_enum;

#[cfg(test)]
mod tests {
    newtype_enum!(
        #[derive(Debug, Clone, Copy)]
        pub enum Test {
            Example1,
            Example2,
        }
    );

    newtype_enum!(
        pub enum Test2 {
            Example1,
            Example2,
        }
    );

    #[rustfmt::skip]
    newtype_enum!(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[derive(strum::EnumIter, strum::EnumString)]
        #[strum(ascii_case_insensitive)]
        pub enum Test3 {
            Example1,
            Example2,
        }
    );

    #[test]
    fn defines_structs() {
        let _ = test::Example1;
        let _ = test::Example2;

        let _ = Test::Example1;
        let _ = Test::Example2;

        let _ = test2::Example1;
        let _ = test2::Example2;

        let _ = Test2::Example1;
        let _ = Test2::Example2;
    }
}
