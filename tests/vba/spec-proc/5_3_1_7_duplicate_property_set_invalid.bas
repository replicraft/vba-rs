' INV: vbal.5.3.1.7
' SOURCE: spec ms-vbal 5.3.1.7 each Property Set in a module has a different name
' ERROR: load
'#MODULE Cls class
Public Property Set P(ByVal v As Object)
End Property
Public Property Set P(ByVal v As Object)
End Property
'#MODULE Module1 standard
Public Sub Main()
End Sub
