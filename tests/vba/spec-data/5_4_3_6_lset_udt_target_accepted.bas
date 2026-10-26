' INV: vbal.5.4.3.6, stmt.lset
' SOURCE: spec ms-vbal 5.4.3.6 UDT is a valid declared type for LSet
' XFAIL: wrong pending
' EXPECT: ok
Private Type T
    a As Long
End Type
Public Sub Main()
    Dim x As T, y As T
    LSet x = y
    Debug.Print "ok"
End Sub
