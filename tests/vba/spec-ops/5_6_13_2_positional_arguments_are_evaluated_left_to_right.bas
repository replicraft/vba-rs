' INV: vbal.5.6.13.2
' SOURCE: derived 5.6.13.2 the positional argument queue is evaluated in the order written
' METHOD: by hand: the trace lines appear in the order a, b, c and the sum is 1 + 2 + 3
' EXPECT: a
' EXPECT: b
' EXPECT: c
' EXPECT:  6 
Private Function T(tag As String, v As Long) As Long
    Debug.Print tag
    T = v
End Function
Private Function Sum3(x As Long, y As Long, z As Long) As Long
    Sum3 = x + y + z
End Function
Public Sub Main()
    Debug.Print Sum3(T("a", 1), T("b", 2), T("c", 3))
End Sub
