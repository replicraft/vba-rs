' INV: vbal.5.2.1.4
' SOURCE: spec ms-vbal 5.2.1.4 Option Private only in procedural modules
' ERROR: load
'#MODULE C class
Option Private Module
Public Sub Hi()
End Sub
'#MODULE Module1 standard
Public Sub Main()
End Sub
