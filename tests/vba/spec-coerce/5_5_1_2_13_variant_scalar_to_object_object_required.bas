' INV: vbal.5.5.1.2.13, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.13 Let-coercion to and from a class, a non-object value (via Variant) to a class or Object is Object required
' XFAIL: wrong pending
' ERROR: 424
Public Sub Main()
    Dim v As Variant
    Dim o As Object
    v = 5
    o = v
    Debug.Print "not reached"
End Sub
