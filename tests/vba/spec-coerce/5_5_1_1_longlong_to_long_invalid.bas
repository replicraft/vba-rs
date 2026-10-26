' INV: vbal.5.5.1.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.1 Let-coercion static semantics, implicit LongLong to any type other than LongLong or Variant is invalid
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
    Dim q As LongLong
    Dim l As Long
    q = 5
    l = q
End Sub
