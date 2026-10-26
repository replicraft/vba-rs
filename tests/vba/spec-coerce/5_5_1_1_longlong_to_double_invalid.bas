' INV: vbal.5.5.1.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.1 Let-coercion static semantics, implicit LongLong to Double is invalid
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
    Dim q As LongLong
    Dim d As Double
    q = 5
    d = q
End Sub
