use nom::{
    error::{ErrorKind, ParseError},
    Check, Err, Input, Mode, OutputM, OutputMode, PResult, Parser,
};

/// it's [`nom::separated_list1`] except it fails if the first separator isn't found
pub fn separated_list1_nonoptional<I, E, F, G>(
    separator: G,
    parser: F,
) -> impl Parser<I, Output = Vec<<F as Parser<I>>::Output>, Error = E>
where
    I: Clone + Input,
    F: Parser<I, Error = E>,
    G: Parser<I, Error = E>,
    E: ParseError<I>,
{
    SeparatedList1 { parser, separator }
}

struct SeparatedList1<F, G> {
    parser: F,
    separator: G,
}

impl<I, E: ParseError<I>, F, G> Parser<I> for SeparatedList1<F, G>
where
    I: Clone + Input,
    F: Parser<I, Error = E>,
    G: Parser<I, Error = E>,
{
    type Output = Vec<<F as Parser<I>>::Output>;
    type Error = <F as Parser<I>>::Error;
    fn process<OM: OutputMode>(&mut self, mut i: I) -> PResult<OM, I, Self::Output, Self::Error> {
        let mut res = OM::Output::bind(Vec::new);

        match self.parser.process::<OM>(i.clone()) {
            Err(e) => return Err(e),
            Ok((i1, o)) => {
                res = OM::Output::combine(res, o, |mut res, o| {
                    res.push(o);
                    res
                });
                i = i1;
            }
        }

        loop {
            let len = i.input_len();
            match self
                .separator
                .process::<OutputM<Check, Check, OM::Incomplete>>(i.clone())
            {
                Err(Err::Error(_)) => {
                    return Err(Err::Error(OM::Error::bind(|| {
                        <F as Parser<I>>::Error::from_error_kind(i, ErrorKind::SeparatedList)
                    })))
                }

                Err(Err::Failure(e)) => return Err(Err::Failure(e)),
                Err(Err::Incomplete(e)) => return Err(Err::Incomplete(e)),
                Ok((i1, _)) => {
                    match self
                        .parser
                        .process::<OutputM<OM::Output, Check, OM::Incomplete>>(i1.clone())
                    {
                        Err(Err::Error(_)) => return Ok((i, res)),
                        Err(Err::Failure(e)) => return Err(Err::Failure(e)),
                        Err(Err::Incomplete(e)) => return Err(Err::Incomplete(e)),
                        Ok((i2, o)) => {
                            // infinite loop check: the parser must always consume
                            if i2.input_len() == len {
                                return Err(Err::Error(OM::Error::bind(|| {
                                    <F as Parser<I>>::Error::from_error_kind(
                                        i,
                                        ErrorKind::SeparatedList,
                                    )
                                })));
                            }

                            res = OM::Output::combine(res, o, |mut res, o| {
                                res.push(o);
                                res
                            });
                            i = i2;
                        }
                    }
                }
            }
        }
    }
}
