' INV: stmt.implements
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/implements-statement
' ERROR: load
'#MODULE IShape class
Public Function Area() As Double
End Function
'#MODULE Bad class
Implements IShape
'#MODULE Module1 standard
Public Sub Main()
End Sub
