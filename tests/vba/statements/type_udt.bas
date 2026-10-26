' INV: stmt.type, type.udt
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/type-statement
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
