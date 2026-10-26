' INV: vbal.5.4.3.6, stmt.lset
' SOURCE: derived ms-vbal 5.4.3.6 LSet copies the bytes of a user-defined type, so equal member layouts copy member by member; numbers print with a leading sign space
' EXPECT:  7 abc
' EXPECT:  7 abc
Private Type A
    n As Long
    s As String * 3
End Type
Private Type B
    m As Long
    t As String * 3
End Type
Public Sub Main()
    Dim x As A, y As A, z As B
    x.n = 7
    x.s = "abc"
    LSet y = x
    Debug.Print y.n; y.s
    LSet z = x
    Debug.Print z.m; z.t
End Sub
