model ListItem {
  i: uint8;
}

model NestedType {
  city: string;
}

model IncludedType {
  nested: NestedType;
}

@portmapping
model A {
  name: Record<string>;
  age: uint8;
  b: B;
  bs: Array<Array<B>>;
}

model B {
  name: Record<string>;
  age: uint8;
}

model C<T> {
  a: Array<integer>;
  b: B;
  t: T;
  included: IncludedType;
  ol: Array<ListItem>;
}

interface TestNamespace {
  foo(a: integer, b: string): A;
  bar(): B;
  z<T>(c: C<T>): C<T>;
}
