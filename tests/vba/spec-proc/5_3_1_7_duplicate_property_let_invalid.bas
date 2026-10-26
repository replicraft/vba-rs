' INV: vbal.5.3.1.7
' SOURCE: spec ms-vbal 5.3.1.7 each Property Let in a module has a different name
' ERROR: load
'#MODULE Cls class
Public Property Let P(ByVal v As Long)
End Property
Public Property Let P(ByVal v As Long)
End Property
'#MODULE Module1 standard
Public Sub Main()
End Sub
