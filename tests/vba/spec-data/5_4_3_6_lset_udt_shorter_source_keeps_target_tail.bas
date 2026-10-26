' INV: vbal.5.4.3.6, stmt.lset
' SOURCE: unverified LSet between user-defined types of different sizes copies as many bytes as the shorter one has; that the rest of a longer target keeps its value is assumed, not documented
' EXPECT: 5 9
' EXPECT: 5 9
Private Type Small
    a As Long
End Type
Private Type Big
    b As Long
    c As Long
End Type
Public Sub Main()
    Dim s As Small, g As Big
    s.a = 5
    g.b = 1
    g.c = 9
    LSet g = s
    Debug.Print CStr(g.b) & " " & CStr(g.c)
    Dim t As Big, u As Small
    t.b = 5
    t.c = 77
    LSet u = t
    g.b = 0
    LSet g = u
    Debug.Print CStr(g.b) & " " & CStr(g.c)
End Sub
