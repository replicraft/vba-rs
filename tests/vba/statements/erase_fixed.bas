' INV: stmt.erase
' SOURCE: spec ms-vbal 5.4.3.4 Erase statement on a fixed-size array
' EXPECT:  0 
Public Sub Main()
    Dim a(1 To 3) As Long
    a(2) = 9
    Erase a
    Debug.Print a(2)
End Sub
