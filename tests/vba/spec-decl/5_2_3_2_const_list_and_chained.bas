' INV: vbal.5.2.3.2
' SOURCE: spec ms-vbal 5.2.3.2 const-item-list, constant-expression referencing earlier constants
' EXPECT: 2 6 7
Const A = 2, B = A * 3
Public Const C As Long = B + 1
Public Sub Main()
    Debug.Print CStr(A) & " " & CStr(B) & " " & CStr(C)
End Sub
