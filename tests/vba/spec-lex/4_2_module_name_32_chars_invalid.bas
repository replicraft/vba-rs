' INV: vbal.4.2
' SOURCE: spec ms-vbal 4.2 modules, the maximum length of a module name is 31 characters
' ERROR: load
' STRICT_SPEC: true
'#MODULE Abcdefghij1234567890abcdefghijkl standard
Public Sub Hello()
End Sub
'#MODULE Module1 standard
Public Sub Main()
End Sub
