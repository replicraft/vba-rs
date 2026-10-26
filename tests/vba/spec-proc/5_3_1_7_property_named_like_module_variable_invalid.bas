' INV: vbal.5.3.1.7
' SOURCE: spec ms-vbal 5.3.1.7 property name differs from module variable names
' ERROR: load
'#MODULE Cls class
Private P As Long
Public Property Get P() As Long
End Property
'#MODULE Module1 standard
Public Sub Main()
End Sub
