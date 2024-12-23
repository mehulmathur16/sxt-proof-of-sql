use super::{PostprocessingError, PostprocessingResult, PostprocessingStep};
use crate::base::{
    database::{
        order_by_util::compare_indexes_by_owned_columns_with_direction, OwnedColumn, OwnedTable,
    },
    math::permutation::Permutation,
    scalar::Scalar,
};
use alloc::{string::ToString, vec::Vec};
use proof_of_sql_parser::intermediate_ast::{OrderBy, OrderByDirection};
use serde::{Deserialize, Serialize};

/// A node representing a list of `OrderBy` expressions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderByPostprocessing {
    by_exprs: Vec<OrderBy>,
}

impl OrderByPostprocessing {
    /// Create a new `OrderByPostprocessing` node.
    #[must_use]
    pub fn new(by_exprs: Vec<OrderBy>) -> Self {
        Self { by_exprs }
    }
}

impl<S: Scalar> PostprocessingStep<S> for OrderByPostprocessing {
    /// Apply the slice transformation to the given `OwnedTable`.
    fn apply(&self, owned_table: OwnedTable<S>) -> PostprocessingResult<OwnedTable<S>> {
        // Evaluate the columns by which we order
        // Once we allow OrderBy for general aggregation-free expressions here we will need to call eval()
        let order_by_pairs: Vec<(OwnedColumn<S>, OrderByDirection)> = self
            .by_exprs
            .iter()
            .map(
                |order_by| -> PostprocessingResult<(OwnedColumn<S>, OrderByDirection)> {
                    let identifier: sqlparser::ast::Ident = order_by.expr.into();
                    Ok((
                        owned_table
                            .inner_table()
                            .get(&identifier)
                            .ok_or(PostprocessingError::ColumnNotFound {
                                column: order_by.expr.to_string(),
                            })?
                            .clone(),
                        order_by.direction,
                    ))
                },
            )
            .collect::<PostprocessingResult<Vec<(OwnedColumn<S>, OrderByDirection)>>>()?;
        // Define the ordering
        let permutation = Permutation::unchecked_new_from_cmp(owned_table.num_rows(), |&a, &b| {
            compare_indexes_by_owned_columns_with_direction(&order_by_pairs, a, b)
        });
        // Apply the ordering
        Ok(
            OwnedTable::<S>::try_from_iter(owned_table.into_inner().into_iter().map(
                |(identifier, column)| {
                    (
                        identifier,
                        column
                            .try_permute(&permutation)
                            .expect("There should be no column length mismatch here"),
                    )
                },
            ))
            .expect("There should be no column length mismatch here"),
        )
    }
}
