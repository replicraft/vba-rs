' INV: vbal.5.3.1.7
' SOURCE: spec ms-vbal 5.3.1.7 property declarations sharing a name have equivalent parameter lists
' ERROR: load
'#MODULE Cls class
Public Property Get P(ByVal i As Long) As Long
End Property
Public Property Let P(ByVal v As Long)
End Property
'#MODULE Module1 standard
Public Sub Main()
End Sub
