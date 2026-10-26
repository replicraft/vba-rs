' INV: expr.addressof, stmt.declare
' SOURCE: unverified SetTimer with a TIMERPROC is delivered by the emulated message pump at DoEvents; time is the manual clock moved by Sleep
' EXPECT: 0
' EXPECT: 1
' EXPECT: 1
' EXPECT: 1
' GRANT: native-declare
Private Declare PtrSafe Function SetTimer Lib "user32" (ByVal hwnd As LongPtr, ByVal id As LongPtr, ByVal ms As Long, ByVal proc As LongPtr) As LongPtr
Private Declare PtrSafe Function KillTimer Lib "user32" (ByVal hwnd As LongPtr, ByVal id As LongPtr) As Long
Private Declare PtrSafe Sub Sleep Lib "kernel32" (ByVal ms As Long)
Dim ticks As Long
Public Sub Tick(ByVal hwnd As LongPtr, ByVal msg As Long, ByVal id As LongPtr, ByVal t As Long)
    ticks = ticks + 1
End Sub
Public Sub Main()
    Dim id As LongPtr
    id = SetTimer(0, 0, 100, AddressOf Tick)
    DoEvents
    Debug.Print CStr(ticks)
    Sleep 150
    DoEvents
    Debug.Print CStr(ticks)
    Call KillTimer(0, id)
    Sleep 500
    DoEvents
    Debug.Print CStr(ticks)
    Debug.Print CStr(ticks)
End Sub
