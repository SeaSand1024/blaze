// Copyright 2022 The Blaze Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use datafusion::arrow::array::*;
use datafusion::arrow::datatypes::*;
use datafusion::arrow::error::Result as ArrowResult;
use datafusion::arrow::record_batch::RecordBatch;
use paste::paste;

pub fn new_array_builders(
    schema: &SchemaRef,
    batch_size: usize,
) -> Vec<Box<dyn ArrayBuilder>> {
    pub fn new_array_builder(dt: &DataType, batch_size: usize) -> Box<dyn ArrayBuilder> {
        macro_rules! make_dictionary_builder {
            ($key_type:expr, $value_type:expr) => {{
                make_dictionary_builder!(@match_key: $key_type, $value_type)
            }};
            (@match_key: $key_type:expr, $value_type:expr) => {{
                match $key_type.as_ref() {
                    DataType::Int8 => make_dictionary_builder!(@match_value: Int8, $value_type),
                    DataType::Int16 => make_dictionary_builder!(@match_value: Int16, $value_type),
                    DataType::Int32 => make_dictionary_builder!(@match_value: Int32, $value_type),
                    DataType::Int64 => make_dictionary_builder!(@match_value: Int64, $value_type),
                    DataType::UInt8 => make_dictionary_builder!(@match_value: UInt8, $value_type),
                    DataType::UInt16 => make_dictionary_builder!(@match_value: UInt16, $value_type),
                    DataType::UInt32 => make_dictionary_builder!(@match_value: UInt32, $value_type),
                    DataType::UInt64 => make_dictionary_builder!(@match_value: UInt64, $value_type),
                    _ => unimplemented!("unsupported dictionary key type: {:?}", $key_type),
                }
            }};
            (@match_value: $keyarrowty:ident, $value_type:expr) => {{
                match $value_type.as_ref() {
                    DataType::Int8 => make_dictionary_builder!(@make: $keyarrowty, Int8),
                    DataType::Int16 => make_dictionary_builder!(@make: $keyarrowty, Int16),
                    DataType::Int32 => make_dictionary_builder!(@make: $keyarrowty, Int32),
                    DataType::Int64 => make_dictionary_builder!(@make: $keyarrowty, Int64),
                    DataType::UInt8 => make_dictionary_builder!(@make: $keyarrowty, UInt8),
                    DataType::UInt16 => make_dictionary_builder!(@make: $keyarrowty, UInt16),
                    DataType::UInt32 => make_dictionary_builder!(@make: $keyarrowty, UInt32),
                    DataType::UInt64 => make_dictionary_builder!(@make: $keyarrowty, UInt64),
                    DataType::Float32 => make_dictionary_builder!(@make: $keyarrowty, Float32),
                    DataType::Float64 => make_dictionary_builder!(@make: $keyarrowty, Float64),
                    DataType::Date32 => make_dictionary_builder!(@make: $keyarrowty, Date32),
                    DataType::Date64 => make_dictionary_builder!(@make: $keyarrowty, Date64),
                    DataType::Utf8 | DataType::LargeUtf8 => {
                        make_dictionary_builder!(@make_str: $keyarrowty)
                    }
                    _ => unimplemented!("dictionary value type not supported: {:?}", $value_type),
                }
            }};
            (@make: $keyarrowty:ident, $valuearrowty:ident) => {{
                type KeyBuilder = paste! {[< $keyarrowty Builder >]};
                type ValueBuilder = paste! {[< $valuearrowty Builder >]};
                Box::new(PrimitiveDictionaryBuilder::new(
                    KeyBuilder::new(0),
                    ValueBuilder::new(batch_size),
                ))
            }};
            (@make_str: $keyarrowty:ident) => {{
                type KeyBuilder = paste! {[< $keyarrowty Builder >]};
                Box::new(StringDictionaryBuilder::new(
                    KeyBuilder::new(0),
                    StringBuilder::new(batch_size),
                ))
            }};
        }

        match dt {
            DataType::Dictionary(key_type, value_type) => {
                make_dictionary_builder!(key_type, value_type)
            }
            dt => make_builder(dt, batch_size),
        }
    }

    schema
        .fields()
        .iter()
        .map(|field| new_array_builder(field.data_type(), batch_size))
        .collect::<Vec<_>>()
}

pub fn make_batch(
    schema: SchemaRef,
    mut arrays: Vec<Box<dyn ArrayBuilder>>,
) -> ArrowResult<RecordBatch> {
    let columns = arrays.iter_mut().map(|array| array.finish()).collect();
    RecordBatch::try_new(schema, columns)
}

pub fn builder_extend(
    builder: &mut Box<dyn ArrayBuilder>,
    array: &ArrayRef,
    indices: &[usize],
    data_type: &DataType,
) {
    macro_rules! append_simple {
        ($arrowty:ident) => {{
            type B = paste::paste! {[< $arrowty Builder >]};
            type A = paste::paste! {[< $arrowty Array >]};
            let t = builder.as_any_mut().downcast_mut::<B>().unwrap();
            let f = array.as_any().downcast_ref::<A>().unwrap();
            for &i in indices {
                if f.is_valid(i) {
                    t.append_value(f.value(i)).unwrap();
                } else {
                    t.append_null().unwrap();
                }
            }
        }};
    }

    macro_rules! append_dict {
        ($key_type:expr, $value_type:expr) => {{
            append_dict!(@match_key: $key_type, $value_type)
        }};
        (@match_key: $key_type:expr, $value_type:expr) => {{
            match $key_type.as_ref() {
                DataType::Int8 => append_dict!(@match_value: Int8, $value_type),
                DataType::Int16 => append_dict!(@match_value: Int16, $value_type),
                DataType::Int32 => append_dict!(@match_value: Int32, $value_type),
                DataType::Int64 => append_dict!(@match_value: Int64, $value_type),
                DataType::UInt8 => append_dict!(@match_value: UInt8, $value_type),
                DataType::UInt16=> append_dict!(@match_value: UInt16, $value_type),
                DataType::UInt32 => append_dict!(@match_value: UInt32, $value_type),
                DataType::UInt64 => append_dict!(@match_value: UInt64, $value_type),
                _ => unimplemented!("dictionary key type not supported: {:?}", $value_type),
            }
        }};
        (@match_value: $keyarrowty:ident, $value_type:expr) => {{
            match $value_type.as_ref() {
                DataType::Int8 => append_dict!(@prim: $keyarrowty, Int8),
                DataType::Int16 => append_dict!(@prim: $keyarrowty, Int16),
                DataType::Int32 => append_dict!(@prim: $keyarrowty, Int32),
                DataType::Int64 => append_dict!(@prim: $keyarrowty, Int64),
                DataType::UInt8 => append_dict!(@prim: $keyarrowty, UInt8),
                DataType::UInt16 => append_dict!(@prim: $keyarrowty, UInt16),
                DataType::UInt32 => append_dict!(@prim: $keyarrowty, UInt32),
                DataType::UInt64 => append_dict!(@prim: $keyarrowty, UInt64),
                DataType::Float32 => append_dict!(@prim: $keyarrowty, Float32),
                DataType::Float64 => append_dict!(@prim: $keyarrowty, Float64),
                DataType::Date32 => append_dict!(@prim: $keyarrowty, Date32),
                DataType::Date64 => append_dict!(@prim: $keyarrowty, Date64),
                DataType::Utf8 => append_dict!(@str: $keyarrowty, i32),
                DataType::LargeUtf8 => append_dict!(@str: $keyarrowty, i64),
                _ => unimplemented!("dictionary value type not supported: {:?}", $value_type),
            }
        }};
        (@prim: $keyarrowty:ident, $valuearrowty:ident) => {{
            type KeyType = paste! {[< $keyarrowty Type >]};
            type ValueType = paste! {[< $valuearrowty Type >]};
            type B = PrimitiveDictionaryBuilder<KeyType, ValueType>;
            type A = DictionaryArray<KeyType>;
            let t = builder.as_any_mut().downcast_mut::<B>().unwrap();
            let f = array.as_any().downcast_ref::<A>().unwrap();
            let fv = f.values().as_any().downcast_ref::<paste! {[<$valuearrowty Array>]} >().unwrap();
            for &i in indices {
                if f.is_valid(i) {
                    t.append(fv.value(f.key(i).unwrap())).unwrap();
                } else {
                    t.append_null().unwrap();
                }
            }
        }};
        (@str: $keyarrowty:ident, $strsizety:ty) => {{
            type KeyType = paste! {[< $keyarrowty Type >]};
            type B = StringDictionaryBuilder<KeyType>;
            type A = DictionaryArray<KeyType>;
            let t = builder.as_any_mut().downcast_mut::<B>().unwrap();
            let f = array.as_any().downcast_ref::<A>().unwrap();
            let fv = f.values().as_any().downcast_ref::<GenericStringArray<$strsizety>>().unwrap();
            for &i in indices {
                if f.is_valid(i) {
                    t.append(fv.value(f.key(i).unwrap())).unwrap();
                } else {
                    t.append_null().unwrap();
                }
            }
        }};
    }

    match data_type {
        DataType::Boolean => append_simple!(Boolean),
        DataType::Int8 => append_simple!(Int8),
        DataType::Int16 => append_simple!(Int16),
        DataType::Int32 => append_simple!(Int32),
        DataType::Int64 => append_simple!(Int64),
        DataType::UInt8 => append_simple!(UInt8),
        DataType::UInt16 => append_simple!(UInt16),
        DataType::UInt32 => append_simple!(UInt32),
        DataType::UInt64 => append_simple!(UInt64),
        DataType::Float32 => append_simple!(Float32),
        DataType::Float64 => append_simple!(Float64),
        DataType::Date32 => append_simple!(Date32),
        DataType::Date64 => append_simple!(Date64),
        DataType::Time32(TimeUnit::Second) => append_simple!(Time32Second),
        DataType::Time32(TimeUnit::Millisecond) => append_simple!(Time32Millisecond),
        DataType::Time64(TimeUnit::Microsecond) => append_simple!(Time64Microsecond),
        DataType::Time64(TimeUnit::Nanosecond) => append_simple!(Time64Nanosecond),
        DataType::Utf8 => append_simple!(String),
        DataType::LargeUtf8 => append_simple!(LargeString),
        DataType::Decimal(_, _) => append_simple!(Decimal),
        DataType::Dictionary(key_type, value_type) => append_dict!(key_type, value_type),
        _ => unimplemented!("data type not supported in shuffle write"),
    }
}

pub fn builder_append_null(to: &mut Box<dyn ArrayBuilder>, data_type: &DataType) {
    macro_rules! append {
        ($arrowty:ident) => {{
            type B = paste::paste! {[< $arrowty Builder >]};
            let t = to.as_any_mut().downcast_mut::<B>().unwrap();
            t.append_null().unwrap();
        }};
    }
    match data_type {
        DataType::Boolean => append!(Boolean),
        DataType::Int8 => append!(Int8),
        DataType::Int16 => append!(Int16),
        DataType::Int32 => append!(Int32),
        DataType::Int64 => append!(Int64),
        DataType::UInt8 => append!(UInt8),
        DataType::UInt16 => append!(UInt16),
        DataType::UInt32 => append!(UInt32),
        DataType::UInt64 => append!(UInt64),
        DataType::Float32 => append!(Float32),
        DataType::Float64 => append!(Float64),
        DataType::Date32 => append!(Date32),
        DataType::Date64 => append!(Date64),
        DataType::Time32(TimeUnit::Second) => append!(Time32Second),
        DataType::Time32(TimeUnit::Millisecond) => append!(Time32Millisecond),
        DataType::Time64(TimeUnit::Microsecond) => append!(Time64Microsecond),
        DataType::Time64(TimeUnit::Nanosecond) => append!(Time64Nanosecond),
        DataType::Utf8 => append!(String),
        DataType::LargeUtf8 => append!(LargeString),
        DataType::Decimal(_, _) => append!(Decimal),
        _ => unimplemented!("data type not supported in sort-merge join"),
    }
}