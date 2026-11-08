//! `eval!` — the way a state machine is tested.
//!
//! A state machine test is a conversation: events go in, actions come out,
//! more events go in. Written by hand that is a wall of `Vec::new()`,
//! `handle_events` and index arithmetic, and the shape of the conversation is
//! the thing that gets lost.
//!
//! ```ignore
//! eval!(&mut dock,
//!     < Event::FromHost(EventHost::Resized { size: Size::new(800.0, 600.0) }),
//!     < Event::FromHost(EventHost::Opened { window: canvas, .. }),
//!
//!     < Event::FromPointer(EventPointer::Down { at: Point::new(12.0, 13.0) }),
//!     < Event::FromPointer(EventPointer::Up { at: Point::new(12.0, 13.0) }),
//!     > Action::ToStore(ActionStore::Save { .. }),
//! );
//! ```
//!
//! `<` is an event going in, `>` is an action expected out. A run of adjacent
//! events goes into one `handle_events` call, the actions are asked for once
//! afterwards with `emit_actions`, and the `>` lines are exactly what that
//! turn must produce — in order, with nothing extra and nothing missing. A `<`
//! after an action starts the next turn.
//!
//! A trailing turn with no `>` asserts silence: the events were accepted and
//! the machine deliberately did nothing.
//!
//! The `>` lines are **patterns**, not values, so `..` and `_` work wherever a
//! part does not matter. That is what keeps this honest without `PartialEq`:
//! the macro expands to a pattern match, which needs no trait at all, and
//! keeping that derive off the events and actions is the point — it spreads
//! through every type they touch.

#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use quote::quote;
use syn::Expr;
use syn::Pat;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse_macro_input;

/// A run of events and the actions they must produce.
struct Batch {
    events: Vec<Expr>,
    actions: Vec<Pat>,
}

struct Eval {
    /// The machine, as `&mut something`.
    state: Expr,
    batches: Vec<Batch>,
}

impl Parse for Eval {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let state: Expr = input.parse()?;
        input.parse::<Token![,]>()?;

        let mut batches: Vec<Batch> = Vec::new();

        while !input.is_empty() {
            if input.peek(Token![<]) {
                input.parse::<Token![<]>()?;
                let event: Expr = input.parse()?;

                // An event after an action starts the next batch.
                let open = matches!(batches.last(), Some(batch) if batch.actions.is_empty());

                if !open {
                    batches.push(Batch {
                        events: Vec::new(),
                        actions: Vec::new(),
                    });
                }

                match batches.last_mut() {
                    Some(batch) => batch.events.push(event),
                    None => unreachable!("a batch was just pushed"),
                }
            } else if input.peek(Token![>]) {
                input.parse::<Token![>]>()?;

                // A pattern rather than an expression, so `..` and `_` work.
                let action = Pat::parse_multi_with_leading_vert(input)?;

                match batches.last_mut() {
                    Some(batch) => batch.actions.push(action),
                    None => {
                        return Err(input.error("an action needs at least one `<` event first"));
                    }
                }
            } else {
                return Err(input.error("expected `<` for an event or `>` for an action"));
            }

            if input.is_empty() {
                break;
            }

            input.parse::<Token![,]>()?;
        }

        Ok(Eval { state, batches })
    }
}

/// Drive a state machine and check what it does. See the module docs.
#[proc_macro]
pub fn eval(input: TokenStream) -> TokenStream {
    let Eval { state, batches } = parse_macro_input!(input as Eval);

    let steps = batches.iter().enumerate().map(|(i, batch)| {
        let events = &batch.events;
        let batch_number = i + 1;
        let expected = batch.actions.len();

        // One check per action, in order. Written out rather than looped so a
        // failure names the position and prints the pattern that did not match.
        let checks = batch.actions.iter().enumerate().map(|(j, action)| {
            let position = j + 1;
            // `quote` puts a space around every token, so the pattern in a
            // failure message would read `Action :: ToUi { action : … }`.
            // Tightening the punctuation costs nothing and the message is the
            // only thing anyone sees when a test breaks.
            let pattern = quote!(#action)
                .to_string()
                .replace(" :: ", "::")
                .replace(" : ", ": ")
                .replace(" , ", ", ");

            quote! {
                match __actions.get(#j) {
                    Some(__action) => assert!(
                        ::std::matches!(__action, #action),
                        "batch {}, action {}: expected `{}`, got {:#?}",
                        #batch_number,
                        #position,
                        #pattern,
                        __action,
                    ),
                    None => unreachable!("the count was checked first"),
                }
            }
        });

        quote! {
            {
                let __events = vec![#(#events),*];

                let mut __actions = ::std::vec::Vec::new();
                __eval_state.handle_events(&__events);
                __eval_state.emit_actions(&mut __actions);

                // Checked before the patterns so that a missing or extra action
                // reports as that, rather than as a mismatch three lines down.
                assert_eq!(
                    __actions.len(),
                    #expected,
                    "batch {} produced {} actions, expected {}: {:#?}",
                    #batch_number,
                    __actions.len(),
                    #expected,
                    __actions,
                );

                #(#checks)*
            }
        }
    });

    let expanded = quote! {
        {
            let __eval_state = #state;

            #(#steps)*
        }
    };

    expanded.into()
}
