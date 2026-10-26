' INV: stmt.implements, expr.typeof
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/implements-statement
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
