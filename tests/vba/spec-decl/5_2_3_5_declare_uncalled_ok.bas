' INV: vbal.5.2.3.5
' SOURCE: spec ms-vbal 5.2.3.5 external procedure declarations (never called) are valid declarations
' EXPECT: ok
Private Declare PtrSafe Function GetTick Lib "kernel32" Alias "GetTickCount" () As Long
Public Declare PtrSafe Sub Pause Lib "kernel32" Alias "Sleep" (ByVal ms As Long)
Declare PtrSafe Function Ordinal Lib "somelib" Alias "#12" (ByVal n As Long) As Long
Public Sub Main()
    Debug.Print "ok"
End Sub
