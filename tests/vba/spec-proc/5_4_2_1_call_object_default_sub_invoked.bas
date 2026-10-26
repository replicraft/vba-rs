' INV: vbal.5.4.2.1, stmt.call
' SOURCE: spec ms-vbal 5.4.2.1 Call on an object with a default subroutine invokes it with the argument list
' EXPECT: default 7
'#MODULE Runner class
Public Sub Run(ByVal n As Long)
Attribute Run.VB_UserMemId = 0
    Debug.Print "default " & CStr(n)
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim o As Object
    Set o = New Runner
    Call o(7)
End Sub
