' INV: stmt.implements
' SOURCE: spec ms-vbal 5.2.4.2 Implements directive requires an implemented name declaration per public method
' ERROR: load
'#MODULE IShape class
Public Function Area() As Double
End Function
'#MODULE Bad class
Implements IShape
'#MODULE Module1 standard
Public Sub Main()
End Sub
