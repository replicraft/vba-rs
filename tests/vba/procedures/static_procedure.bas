' INV: stmt.static, stmt.sub
' SOURCE: spec ms-vbal 5.3.1.2 Static procedures
' EXPECT:  1 
' EXPECT:  2 
Static Sub Count()
    Dim n As Long
    n = n + 1
    Debug.Print n
End Sub
Public Sub Main()
    Count
    Count
End Sub
