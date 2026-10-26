' INV: vbal.5.4.3.1, stmt.dim
' SOURCE: spec ms-vbal 5.4.3.1 Dim variables have procedure extent
' EXPECT: 1
' EXPECT: 1
Private Sub Counter()
    Dim n As Integer
    n = n + 1
    Debug.Print CStr(n)
End Sub
Public Sub Main()
    Counter
    Counter
End Sub
