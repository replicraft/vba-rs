' INV: op.concat, lit.empty
' SOURCE: derived Empty is a zero-length string for &
' EXPECT: x
Public Sub Main()
    Debug.Print CStr(Empty & "x")
End Sub
