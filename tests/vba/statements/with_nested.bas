' INV: stmt.with, expr.with-member
' SOURCE: spec ms-vbal 5.4.2.21 With statement, 5.6.15 With expressions
' EXPECT:  1  2 
Type Inner
    N As Long
End Type
Type Outer
    I As Inner
    M As Long
End Type
Public Sub Main()
    Dim o As Outer
    With o
        .M = 1
        With .I
            .N = 2
        End With
    End With
    Debug.Print o.M; o.I.N
End Sub
