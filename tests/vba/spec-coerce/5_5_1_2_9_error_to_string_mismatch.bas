' INV: vbal.5.5.1.2.9, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.9 Let-coercion to and from Error, an Error value to String is Type mismatch
' XFAIL: wrong pending
' ERROR: 13
Public Sub Main()
    Dim v As Variant
    Dim s As String
    v = CVErr(7)
    s = v
    Debug.Print "not reached"
End Sub
