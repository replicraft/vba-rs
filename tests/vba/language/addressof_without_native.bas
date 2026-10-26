' INV: expr.addressof
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/declare-statement
' EXPECT: True
' XFAIL: unsupported AddressOf produces no callable native pointer without the Declare bridge
Function Cb() As Long
    Cb = 1
End Function
Public Sub Main()
    Dim p As LongPtr
    p = AddressOf Cb
    Debug.Print p <> 0
End Sub
