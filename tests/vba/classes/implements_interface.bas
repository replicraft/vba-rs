' INV: stmt.implements
' SOURCE: spec ms-vbal 5.3.1.9 implemented name declarations dispatch, 5.2.4.2 Implements directive
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
