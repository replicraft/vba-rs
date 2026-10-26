' INV: vbal.5.5.1.2.9, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.9 Let-coercion to and from Error, an Error value to any type except Variant is Type mismatch
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13
' EXPECT: iserror=True
' ERROR: 13
Public Sub Main()
    Dim v As Variant
    Dim n As Long
    v = CVErr(5)
    Debug.Print "iserror=" & CStr(IsError(v))
    n = v
    Debug.Print "not reached"
End Sub
