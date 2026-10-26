' INV: vbal.2.3
' SOURCE: spec ms-vbal 2.3 variables, procedure-extent variables are created per invocation; static locals have module extent
' EXPECT: local=1
' EXPECT: local=1
' EXPECT: static=1
' EXPECT: static=2
' EXPECT: module=1
' EXPECT: module=2
Private mCount As Long
Private Sub Local()
    Dim n As Long
    n = n + 1
    Debug.Print "local=" & n
End Sub
Private Sub StaticLocal()
    Static n As Long
    n = n + 1
    Debug.Print "static=" & n
End Sub
Private Sub Module()
    mCount = mCount + 1
    Debug.Print "module=" & mCount
End Sub
Public Sub Main()
    Local
    Local
    StaticLocal
    StaticLocal
    Module
    Module
End Sub
