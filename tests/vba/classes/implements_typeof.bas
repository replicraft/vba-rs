' INV: stmt.implements, expr.typeof
' SOURCE: spec ms-vbal 5.6.7 TypeOf...Is expression on an implementing class
' EXPECT: True
'#MODULE IShape class
Public Sub Draw()
End Sub
'#MODULE Sq class
Implements IShape
Private Sub IShape_Draw()
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim o As Object
    Set o = New Sq
    Debug.Print TypeOf o Is IShape
End Sub
