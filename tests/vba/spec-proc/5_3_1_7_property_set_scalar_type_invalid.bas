' INV: vbal.5.3.1.7
' SOURCE: spec ms-vbal 5.3.1.7 Property Set declared type must be Object, Variant or a class
' ERROR: load
'#MODULE Cls class
Public Property Set P(ByVal v As Long)
End Property
'#MODULE Module1 standard
Public Sub Main()
End Sub
