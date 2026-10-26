' INV: vbal.5.4.1.2
' SOURCE: spec ms-vbal 5.4.1.2 Rem statements have no effect
' EXPECT: n=2
Rem a whole-line remark Debug.Print "no"
Public Sub Main()
    Dim n As Long
    n = 1
    Rem n = 100
    n = n + 1: Rem n = n + 50
    Debug.Print "n=" & CStr(n)
End Sub
