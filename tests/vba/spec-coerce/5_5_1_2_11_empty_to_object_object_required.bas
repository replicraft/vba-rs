' INV: vbal.5.5.1.2.11, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.11 Let-coercion from Empty, Empty to a class or Object is Object required
' XFAIL: wrong pending
' ERROR: 424
Public Sub Main()
    Dim v As Variant
    Dim o As Object
    o = v
    Debug.Print "not reached"
End Sub
