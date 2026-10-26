' INV: vbal.5.3.1.7
' SOURCE: spec ms-vbal 5.3.1.7 each Property Get in a module has a different name
' ERROR: load
'#MODULE Cls class
Public Property Get P() As Long
End Property
Public Property Get P() As Long
End Property
'#MODULE Module1 standard
Public Sub Main()
End Sub
