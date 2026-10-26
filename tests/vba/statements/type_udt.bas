' INV: stmt.type, type.udt
' SOURCE: spec ms-vbal 5.2.3.3 User Defined Type declarations, 5.5.1.2.8 UDT Let-coercion copy
' EXPECT:  1  2  9 
Type Point
    X As Long
    Y As Long
End Type
Public Sub Main()
    Dim p As Point, q As Point
    p.X = 1: p.Y = 2
    q = p
    q.X = 9
    Debug.Print p.X; p.Y; q.X
End Sub
