' INV: vbal.6.1.1.2
' SOURCE: spec ms-vbal 6.1.1.2 VbAppWinStyle values
' XFAIL: wrong pending
' EXPECT: vbHide=0
' EXPECT: vbMaximizedFocus=3
' EXPECT: vbMinimizedFocus=2
' EXPECT: vbMinimizedNoFocus=6
' EXPECT: vbNormalFocus=1
' EXPECT: vbNormalNoFocus=4
Public Sub Main()
    Debug.Print "vbHide=" & CStr(vbHide)
    Debug.Print "vbMaximizedFocus=" & CStr(vbMaximizedFocus)
    Debug.Print "vbMinimizedFocus=" & CStr(vbMinimizedFocus)
    Debug.Print "vbMinimizedNoFocus=" & CStr(vbMinimizedNoFocus)
    Debug.Print "vbNormalFocus=" & CStr(vbNormalFocus)
    Debug.Print "vbNormalNoFocus=" & CStr(vbNormalNoFocus)
End Sub
