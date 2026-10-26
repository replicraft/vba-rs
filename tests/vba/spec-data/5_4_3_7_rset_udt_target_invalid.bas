' INV: vbal.5.4.3.7, stmt.rset
' SOURCE: spec ms-vbal 5.4.3.7 UDT is not allowed for RSet (unlike LSet)
' ERROR: load
Private Type T
    a As Long
End Type
Public Sub Main()
    Dim x As T, y As T
    RSet x = y
End Sub
