' INV: vbal.6.1.2.3.1.17, func.hex
' SOURCE: spec ms-vbal 6.1.2.3.1.17 Hex of a Long -1 is judged by the coerced value range (four characters)
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: Hex of a negative Long has 8 digits
' EXPECT: long=FFFF
Public Sub Main()
    Dim n As Long
    n = -1
    Debug.Print "long=" & Hex(n)
End Sub
