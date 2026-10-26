' INV: vbal.5.5.1.2.9, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.9 Let-coercion to and from Error, an Error value to String is Type mismatch
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13
' ERROR: 13
Public Sub Main()
    Dim v As Variant
    Dim s As String
    v = CVErr(7)
    s = v
    Debug.Print "not reached"
End Sub
