' INV: vbal.6.1.2.3.1.7, func.cint
' SOURCE: spec ms-vbal 6.1.2.3.1.7 CInt of an Error value whose code exceeds Integer range overflows
' XFAIL: wrong pending
' EXPECT: err40000=6
Public Sub Main()
    Dim v As Variant
    On Error Resume Next
    Err.Clear: v = CInt(CVErr(40000)): Debug.Print "err40000=" & Err.Number
End Sub
