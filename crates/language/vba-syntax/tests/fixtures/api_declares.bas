Attribute VB_Name = "WinApi"
Option Explicit

#If VBA7 Then
    Private Declare PtrSafe Function GetTickCount Lib "kernel32" () As Long
    Private Declare PtrSafe Sub Sleep Lib "kernel32" (ByVal dwMilliseconds As Long)
    Private Declare PtrSafe Function FindWindow Lib "user32" Alias "FindWindowA" _
        (ByVal lpClassName As String, ByVal lpWindowName As String) As LongPtr
    Private Declare PtrSafe Function SendMessage Lib "user32" Alias "SendMessageA" _
        (ByVal hWnd As LongPtr, ByVal wMsg As Long, ByVal wParam As LongPtr, lParam As Any) As LongPtr
    Private Declare PtrSafe Function GetUserName Lib "advapi32.dll" Alias "GetUserNameA" _
        (ByVal lpBuffer As String, nSize As Long) As Long
    #If Win64 Then
        Private Declare PtrSafe Function GetTickCount64 Lib "kernel32" () As LongLong
        Private Type POINTAPI
            x As Long
            y As Long
        End Type
    #End If
    Public Declare PtrSafe Function CopyMem Lib "kernel32" Alias "RtlMoveMemory" _
        (Destination As Any, Source As Any, ByVal Length As LongPtr) As Long
#Else
    Private Declare Function GetTickCount Lib "kernel32" () As Long
    Private Declare Sub Sleep Lib "kernel32" (ByVal dwMilliseconds As Long)
    Private Declare Function FindWindow Lib "user32" Alias "FindWindowA" _
        (ByVal lpClassName As String, ByVal lpWindowName As String) As Long
#End If

#If Win64 Then
    Private Const PTR_SIZE As Long = 8
#Else
    Private Const PTR_SIZE As Long = 4
#End If

Public Function UserName() As String
    Dim buf As String * 255
    Dim n As Long
    n = 255
    If GetUserName(buf, n) <> 0 Then
        UserName = Left$(buf, n - 1)
    End If
End Function

Public Sub Pause(ByVal ms As Long)
    Dim t As Long
    t = GetTickCount()
    Do While GetTickCount() - t < ms
        DoEvents
        Sleep 10
    Loop
End Sub

Public Function Callback(ByVal hWnd As LongPtr, ByVal lParam As LongPtr) As Long
    Callback = 1
End Function

Public Function HookAddress() As LongPtr
    HookAddress = AddressOf Callback
End Function
