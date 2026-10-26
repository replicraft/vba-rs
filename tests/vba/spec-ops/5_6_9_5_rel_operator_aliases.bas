' INV: vbal.5.6.9.5.2, op.ne, op.le, op.ge
' SOURCE: spec ms-vbal 5.6.9.5.2 <> Operator alternate spellings >< =< =>
' XFAIL: wrong pending
' EXPECT: True
' EXPECT: False
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Debug.Print CStr(1 >< 2)
    Debug.Print CStr(2 >< 2)
    Debug.Print CStr(2 =< 2)
    Debug.Print CStr(2 => 2)
    Debug.Print CStr(1 =< 2)
    Debug.Print CStr(1 => 2)
End Sub
