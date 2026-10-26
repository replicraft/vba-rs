' INV: vbal.5.4.3.4, stmt.erase
' SOURCE: spec ms-vbal 5.4.3.4 fixed array of Variant elements reset to Empty
' EXPECT: True
Public Sub Main()
    Dim a(1) As Variant
    a(0) = 5
    Erase a
    Debug.Print CStr(IsEmpty(a(0)))
End Sub
