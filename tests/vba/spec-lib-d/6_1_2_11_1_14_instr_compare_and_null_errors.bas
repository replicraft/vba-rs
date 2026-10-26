' INV: vbal.6.1.2.11.1.14, func.instr
' SOURCE: spec ms-vbal 6.1.2.11.1.14 InStr Compare argument; Null start or Null Compare is an error
' EXPECT: bin=0
' EXPECT: txt=2
' EXPECT: nullstart=True
' EXPECT: nullcmp=True
Public Sub Main()
    Debug.Print "bin=" & CStr(InStr(1, "ABC", "b", vbBinaryCompare))
    Debug.Print "txt=" & CStr(InStr(1, "ABC", "b", vbTextCompare))
    Dim n As Long
    On Error Resume Next
    n = InStr(Null, "abc", "b")
    Debug.Print "nullstart=" & CStr(Err.Number <> 0)
    Err.Clear
    n = InStr(1, "abc", "b", Null)
    Debug.Print "nullcmp=" & CStr(Err.Number <> 0)
End Sub
