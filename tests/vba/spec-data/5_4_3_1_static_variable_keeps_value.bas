' INV: vbal.5.4.3.1, stmt.static
' SOURCE: spec ms-vbal 5.4.3.1 static variables have module extent
' EXPECT: 1
' EXPECT: 2
' EXPECT: 3
Private Sub Counter()
    Static n As Integer
    n = n + 1
    Debug.Print CStr(n)
End Sub
Public Sub Main()
    Counter
    Counter
    Counter
End Sub
