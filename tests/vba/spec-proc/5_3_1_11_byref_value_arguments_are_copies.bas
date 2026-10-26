' INV: vbal.5.3.1.11, decl.byref-byval
' SOURCE: spec ms-vbal 5.3.1.11 ByRef parameter with a value/function/property argument gets a local copy
' EXPECT: lit=11
' EXPECT: expr=n:5
' EXPECT: fn=n:5
' EXPECT: paren=n:5
Dim g As Long
Function Five() As Long
    Five = 5
End Function
Sub Bump(ByRef x As Long)
    x = x + 6
End Sub
Public Sub Main()
    Dim n As Long
    n = 5
    Debug.Print "lit=" & CStr(Echo(5))
    Bump n + 0
    Debug.Print "expr=n:" & CStr(n)
    Bump Five()
    Debug.Print "fn=n:" & CStr(Five())
    Bump (n)
    Debug.Print "paren=n:" & CStr(n)
End Sub
Function Echo(ByRef x As Long) As Long
    x = x + 6
    Echo = x
End Function
