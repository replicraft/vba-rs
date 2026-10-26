' INV: vbal.5.2.1.2
' SOURCE: spec ms-vbal 5.2.1.2 Option Base must precede first dim-spec
' ERROR: load
Dim a(3) As Long
Option Base 1
Public Sub Main()
End Sub
