' INV: vbal.5.6.16.1, op.like
' SOURCE: spec ms-vbal 5.6.16.1 constant expressions allow Like, relational and Eqv Imp operators
' XFAIL: wrong pending
' EXPECT: True
' EXPECT: False
' EXPECT: True
' EXPECT: -7
' EXPECT: -5
Private Const A As Boolean = "abc" Like "a*"
Private Const B As Boolean = 3 <> 3
Private Const C As Boolean = 2 >= 1
Private Const D As Long = 12 Eqv 10
Private Const E As Long = 12 Imp 10
Public Sub Main()
    Debug.Print CStr(A)
    Debug.Print CStr(B)
    Debug.Print CStr(C)
    Debug.Print CStr(D)
    Debug.Print CStr(E)
End Sub
