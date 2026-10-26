' INV: vbal.5.4.3.6, stmt.lset
' SOURCE: spec ms-vbal 5.4.3.6 qLength zero truncates to zero characters
' EXPECT: [] 0
Public Sub Main()
    Dim s As String
    LSet s = "abc"
    Debug.Print "[" & s & "] " & CStr(Len(s))
End Sub
