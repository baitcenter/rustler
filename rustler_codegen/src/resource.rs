use ::syntax::ext::base::{Annotatable, ExtCtxt, MacResult, MacEager};
use ::syntax::ast::{Item_, MetaItem, Ident};
use ::syntax::codemap::{Span};
use easy_plugin::{PluginResult};

use ::syntax::util::small_vector::{SmallVector};

pub fn resource_struct_def_decorator(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    let builder = ::aster::AstBuilder::new().span(span);

    let struct_ident = annotatable.clone().expect_item().ident;
    let struct_ident_str = &*struct_ident.name.as_str();
    let type_field_name = builder.id(["_rustler_nif_struct_type_", struct_ident_str].concat());
    let dtor_name_ident = builder.id(["_rustler_nif_struct_type_dtor_", struct_ident_str].concat());

    // Static field for Nif Resource Type (Set in resource_struct_init!)
    push(Annotatable::Item(quote_item!(cx,
        #[allow(non_snake_case)]
        static mut $type_field_name: Option<rustler::resource::NifStructResourceType<$struct_ident>> = None;
    ).unwrap()));

    // Destructor for the type
    push(Annotatable::Item(quote_item!(cx,
        #[allow(non_snake_case)]
        extern "C" fn $dtor_name_ident(env: rustler::wrapper::nif_interface::NIF_ENV, 
                                       obj: rustler::wrapper::nif_interface::NIF_RESOURCE_HANDLE) {
            unsafe { rustler::codegen_runtime::handle_drop_resource_struct_handle::<$struct_ident>(env, obj) }
        }
    ).unwrap()));

    // NifResourceStruct trait implementation
    push(Annotatable::Item(quote_item!(cx,
        impl rustler::resource::NifResourceStruct for $struct_ident {
            fn get_dtor() -> extern "C" fn(_env: rustler::wrapper::nif_interface::NIF_ENV, 
                                           handle: rustler::wrapper::nif_interface::NIF_RESOURCE_HANDLE) {
                $dtor_name_ident
            }
            fn get_type<'a>() -> &'a rustler::resource::NifStructResourceType<Self> {
                unsafe { &$type_field_name }.as_ref().unwrap()
            }
        }
    ).unwrap()));

    /*push(Annotatable::Item(quote_item!(cx,
        impl rustler::NifEncoder for $struct_ident {
            fn encode<'a>(&self, env: &'a rustler::NifEnv) -> NifTerm<'a> {
                
            }
        }
    ).unwrap()));*/
}

easy_plugin! {
    struct Arguments { $struct_ident:ident, $env:ident }

    pub fn resource_struct_init_macro(cx: &mut ExtCtxt, span: Span, arguments: Arguments) -> PluginResult<Box<MacResult>> {
        let builder = ::aster::AstBuilder::new().span(span);

        let env_ident = arguments.env;
        let struct_ident = arguments.struct_ident;
        let struct_ident_str = &*struct_ident.name.as_str();
        let type_field_name_ident = builder.id(["_rustler_nif_struct_type_", struct_ident_str].concat());
        let init_item = quote_stmt!(cx, {
            let res = match rustler::resource::open_struct_resource_type::<$struct_ident>($env_ident, $struct_ident_str, 
                                                                                          rustler::ErlNifResourceFlags::ERL_NIF_RT_CREATE) {
                Some(inner) => inner,
                None => {
                    println!("Failiure in creating resource type");
                    return false;
                }
            };
            unsafe {
                $type_field_name_ident = Some(res);
            };
        }).unwrap();

        Ok(MacEager::stmts(SmallVector::one(init_item)))
    }
}