' INV: stmt.implements
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/implements-statement
' EXPECT:  16 
' EXPECT:  3 
'#MODULE IShape class
Public Function Area() As Double
End Function
'#MODULE Square class
Implements IShape
Public Side As Double
Private Function IShape_Area() As Double
    IShape_Area = Side * Side
End Function
'#MODULE Disc class
Implements IShape
Private Function IShape_Area() As Double
    IShape_Area = 3
End Function
'#MODULE Module1 standard
Public Sub Main()
    Dim s As New Square
    s.Side = 4
    Dim a As IShape
    Set a = s
    Debug.Print a.Area
    Set a = New Disc
    Debug.Print a.Area
End Sub
