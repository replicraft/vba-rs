' INV: vbal.5.4.2.17, stmt.exit
' SOURCE: spec ms-vbal 5.4.2.17 Exit Sub must be inside a subroutine declaration
' ERROR: load
Function F() As Long
    Exit Sub
End Function
Public Sub Main()
End Sub
