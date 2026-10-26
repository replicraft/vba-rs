' INV: vbal.6.1.2.11.1.15, func.instrrev
' SOURCE: spec ms-vbal 6.1.2.11.1.15 InStrRev, Start Null is an error; binary compare by default
' EXPECT: nullstart=True
' EXPECT: bin=0
Public Sub Main()
    Dim n As Variant
    On Error Resume Next
    n = InStrRev("abc", "b", Null)
    Debug.Print "nullstart=" & CStr(Err.Number <> 0)
    On Error GoTo 0
    Debug.Print "bin=" & CStr(InStrRev("ABC", "b"))
End Sub
