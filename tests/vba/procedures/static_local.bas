' INV: stmt.static
' SOURCE: spec ms-vbal 5.4.3.1 Static variables have module extent
' EXPECT:  1 
' EXPECT:  2 
' EXPECT:  3 
Sub Count()
    Static n As Long
    n = n + 1
    Debug.Print n
End Sub
Public Sub Main()
    Count
    Count
    Count
End Sub
