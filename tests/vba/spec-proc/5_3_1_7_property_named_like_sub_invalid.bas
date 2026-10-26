' INV: vbal.5.3.1.7
' SOURCE: spec ms-vbal 5.3.1.7 property name differs from subroutine and function names
' ERROR: load
'#MODULE Cls class
Public Sub P()
End Sub
Public Property Get P() As Long
End Property
'#MODULE Module1 standard
Public Sub Main()
End Sub
