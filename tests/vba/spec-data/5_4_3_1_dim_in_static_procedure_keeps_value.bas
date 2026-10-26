' INV: vbal.5.4.3.1, stmt.dim
' SOURCE: spec ms-vbal 5.4.3.1 locals of a static procedure have module extent
' EXPECT: 1
' EXPECT: 2
Private Static Sub Counter()
    Dim n As Integer
    n = n + 1
    Debug.Print CStr(n)
End Sub
Public Sub Main()
    Counter
    Counter
End Sub
