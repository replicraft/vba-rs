' INV: vbal.6.1.2.3.1.2, func.cbyte
' SOURCE: spec ms-vbal 6.1.2.3.1.2 CByte of an Error value whose code exceeds Byte range overflows
' XFAIL: wrong pending
' EXPECT: err300=6
Public Sub Main()
    Dim v As Variant
    On Error Resume Next
    Err.Clear: v = CByte(CVErr(300)): Debug.Print "err300=" & Err.Number
End Sub
