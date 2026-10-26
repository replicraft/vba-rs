' INV: vbal.5.3.1.7
' SOURCE: spec ms-vbal 5.3.1.7 shared-name properties have the same actual parameter mechanism
' ERROR: load
' STRICT_SPEC: true
'#MODULE Cls class
Public Property Get P(ByVal i As Long) As Long
End Property
Public Property Let P(ByRef i As Long, ByVal v As Long)
End Property
'#MODULE Module1 standard
Public Sub Main()
End Sub
